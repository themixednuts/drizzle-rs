diesel::table! {
    users (id) {
        id -> Integer,
        name -> Text,
        email -> Nullable<Text>,
        age -> Integer,
    }
}

diesel::table! {
    posts (id) {
        id -> Integer,
        title -> Text,
        content -> Nullable<Text>,
        author_id -> Integer,
    }
}

diesel::joinable!(posts -> users (author_id));
diesel::allow_tables_to_appear_in_same_query!(users, posts);
