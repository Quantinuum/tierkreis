// @generated automatically by Diesel CLI.

diesel::table! {
    node_outputs (id) {
        id -> Integer,
        node_state_id -> Integer,
        name -> Text,
        asset_kind -> Text,
        storage_name -> Text,
        asset_key -> Text,
    }
}

diesel::table! {
    node_states (id) {
        id -> Integer,
        run_id -> Text,
        attempt -> Integer,
        node_location -> Text,
        scheduled_time -> Nullable<Timestamp>,
        queued_time -> Nullable<Timestamp>,
        running_time -> Nullable<Timestamp>,
        complete_time -> Nullable<Timestamp>,
        cancelled_time -> Nullable<Timestamp>,
        error_time -> Nullable<Timestamp>,
        cond -> Nullable<Bool>,
        loop_index -> Nullable<Integer>,
        map_size -> Nullable<Integer>,
        map_completed -> Nullable<Binary>,
        error -> Nullable<Text>,
        error_detail -> Nullable<Text>,
    }
}

diesel::table! {
    workflow_run_inputs (id) {
        id -> Integer,
        workflow_run_id -> Text,
        name -> Text,
        asset_kind -> Text,
        storage_name -> Text,
        asset_key -> Text,
    }
}

diesel::table! {
    workflow_runs (id, attempt) {
        id -> Text,
        attempt -> Integer,
        workflow_id -> Text,
        run_metadata -> Binary,
        status -> Nullable<Text>,
        started_time -> Nullable<Timestamp>,
    }
}

diesel::table! {
    workflows (id) {
        id -> Text,
        name -> Nullable<Text>,
        created_time -> Nullable<Timestamp>,
        definition -> Binary,
    }
}

diesel::joinable!(node_outputs -> node_states (node_state_id));
diesel::joinable!(workflow_runs -> workflows (workflow_id));

diesel::allow_tables_to_appear_in_same_query!(
    node_outputs,
    node_states,
    workflow_run_inputs,
    workflow_runs,
    workflows,
);
