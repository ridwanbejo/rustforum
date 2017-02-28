#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate serde_json;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

#[macro_use(bson, doc)]
extern crate bson;
extern crate jsonwebtoken as jwt;
extern crate rustc_serialize;
extern crate rustforum;
extern crate diesel;

use rocket_contrib::{JSON, Value};
use bson::Bson;
use std::sync::Arc;
use jwt::{encode, decode, Header, Algorithm};
use jwt::errors::{Error};
use rocket::Outcome;
use rocket::http::Status;
use rocket::request::{self, Request, FromRequest};
use self::rustforum::*;
use self::rustforum::models::*;
use self::diesel::prelude::*;
use std::time::SystemTime;
use diesel::insert;
use diesel::delete;
use diesel::update;

const SECRET_KEY: &'static str = "rahasia12345";

#[derive(Debug, RustcEncodable, RustcDecodable)]
struct Claims {
    sub: String,
    company: String
}

// Example validation implementation
impl Claims {
    fn is_valid(&self) -> bool {
        if self.company != "Codepolitan" {
            return false;
        }
        // expiration etc

        true
    }
}

struct Token (String);

impl<'a, 'r> FromRequest<'a, 'r> for Token {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Token, ()> {
        let keys: Vec<_> = request.headers().get("x-token").collect();
        
        if keys.len() != 1 {
            return Outcome::Failure((Status::BadRequest, ()));
        }

        let key = keys[0];
        return Outcome::Success(Token(key.to_string()));
    }
}

#[derive(Deserialize)]
struct QuestionItem {
    question_text: String,
    tags: String
}

#[derive(Serialize)]
struct QuestionRow {
    id: i32,
    question_text: String,
    tags: String,
    user_id: String
}


#[get("/question")]
fn list_question(token: Token) -> JSON<Value> {

    let token_data = match decode::<Claims>(&token.0, SECRET_KEY.as_ref(), Algorithm::HS256) {
        Ok(c) => c,
        Err(err) => match err {
            Error::InvalidToken => panic!(),
            _ => panic!()
        }
    };
    
    println!("{:?}", token_data.claims);
    println!("{:?}", token_data.header);
    println!("{:?}", token_data.claims.is_valid());

    use rustforum::schema::questions::dsl::*;

    let connection = establish_connection();

    let results = questions
        .load::<Question>(&connection)
        .expect("Error loading posts");

    println!("Found {} questions", results.len());
    
    let mut rows: Vec<QuestionRow> = vec![];

    for post in results {
        let question = QuestionRow {id: post.id, question_text: post.question_text, tags: post.tags, user_id: post.user_id};
        rows.push(question);
    }

    println!("Rows length: {}", rows.len());
    
    JSON(json!({
        "message": "Getting the question list...",
        "data": rows
    }))
}


#[get("/question/<qid>")]
fn get_question(token: Token, qid: &str) -> JSON<Value> {

    let token_data = match decode::<Claims>(&token.0, SECRET_KEY.as_ref(), Algorithm::HS256) {
        Ok(c) => c,
        Err(err) => match err {
            Error::InvalidToken => panic!(),
            _ => panic!()
        }
    };
    
    println!("{:?}", token_data.claims);
    println!("{:?}", token_data.header);
    println!("{:?}", token_data.claims.is_valid());

    use rustforum::schema::questions::dsl::*;

    let connection = establish_connection();

    println!("{}", qid);

    let row_id = qid.parse::<i32>().unwrap();

    let row = questions
        .find(row_id)
        .first::<Question>(&connection)
        .expect("Error loading posts");

    println!("{}", row.id);
    println!("{}", row.question_text);

	JSON(json!({
        "message": format!("Getting the question with id: {}", qid),
        "data": {
            "id": row.id,
            "question_text": row.question_text,
            "tags":row.tags,
            "user_id": row.user_id
        }
    }))

}

#[post("/question", format = "application/json", data = "<question>")]
fn create_question(token: Token, question: JSON<QuestionItem>) -> JSON<Value> {
    let quest: String = question.0.question_text;
    let tag: String = question.0.tags;
    let now = SystemTime::now();

    use rustforum::schema::questions::dsl::*;
    
    let connection = establish_connection();
    let mut uid = String::new();
    uid.push_str("12345");

    let new_question = NewQuestion { 
        question_text: quest, 
        tags: tag, 
        created_at: SystemTime::now(),
        user_id: uid 
    };

    insert(&new_question)
        .into(questions)
        .execute(&connection)
        .expect("Error saving new question");

    JSON(json!({
        "message": "Create the new question..",
        "data": {
            "question_text": format!("{}", new_question.question_text),
            "tags": format!("{}", new_question.tags)
        }
    }))
}

#[put("/question/<qid>", format = "application/json", data = "<question>")]
fn update_question(token: Token, qid: &str, question: JSON<QuestionItem>) -> JSON<Value> {
    let quest = question.0.question_text;
    let tag = question.0.tags;

    use rustforum::schema::questions::dsl::*;

    let connection = establish_connection();

    let row_id = qid.parse::<i32>().unwrap();
    
    let row = update(
            questions
            .find(row_id)
        )
        .set(question_text.eq(quest))
        .get_result::<Question>(&connection)
        .expect("Error deleting question");

    JSON(json!({
        "message": "Create the new question..",
        "data": {
            "question_text": format!("{}", row.question_text),
            "tags": format!("{}", row.tags)
        }
    }))
}


#[delete("/question/<qid>")]
fn delete_question(qid: &str) -> JSON<Value> {

    use rustforum::schema::questions::dsl::*;

    let connection = establish_connection();

    let row_id = qid.parse::<i32>().unwrap();

    delete(
            questions
            .find(row_id)
        )
        .execute(&connection)
        .expect("Error deleting question");

    JSON(json!({
        "message": format!("Deleting the question with id: {}", qid)
    }))

}

#[get("/")]
fn index() -> JSON<Value> {

    let my_claims = Claims {
        sub: "ridwanbejo@gmail.com".to_owned(),
        company: "Codepolitan".to_owned()
    };

    let token = match encode(Header::default(), &my_claims, SECRET_KEY.as_ref()) {
        Ok(t) => t,
        Err(_) => panic!() // in practice you would return the error
    };

    println!("{:?}", token);

    JSON(json!({
        "message": "Welcome to RustForum API :D"
    }))
}

fn main() {
    rocket::ignite()
    	.mount("/", routes![index, list_question, get_question, create_question, update_question, delete_question])
    	.launch();
}
