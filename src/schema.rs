use diesel::table;

table! {
    todos (id) {
        id -> Integer,
        channel_id -> BigInt,
        todo -> Text,
        creation_date -> Text,
        completion_date -> Nullable<Text>,
    }
}
