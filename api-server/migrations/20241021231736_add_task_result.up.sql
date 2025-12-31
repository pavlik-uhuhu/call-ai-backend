DO $$ BEGIN
    CREATE TYPE task_result_status AS ENUM ('processing', 'ready', 'failed');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS call_metadata (
    id UUID DEFAULT gen_random_uuid() NOT NULL,
    call_id BIGINT NOT NULL,

    performed_at timestamp with time zone NOT NULL,
    uploaded_at timestamp with time zone NOT NULL,

    file_hash text NOT NULL UNIQUE,
    file_url text NOT NULL,
    file_name text NOT NULL,
    duration real NOT NULL,
    left_channel participant_type NOT NULL,
    right_channel participant_type NOT NULL,
    client_name text NOT NULL,
    employee_name text NOT NULL,
    inbound bool NOT NULL,

    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS task (
    id UUID DEFAULT gen_random_uuid() NOT NULL,
    call_metadata_id UUID NOT NULL,
    status task_result_status NOT NULL,
    failed_reason text,
    project_id UUID DEFAULT '00000000-0000-0000-0000-000000000000' NOT NULL,

    PRIMARY KEY (id),
    FOREIGN KEY (call_metadata_id) REFERENCES call_metadata(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS task_to_dict (
    task_id UUID NOT NULL,
    dictionary_id SERIAL NOT NULL,
    contains bool NOT NULL,

    PRIMARY KEY (task_id, dictionary_id),
    FOREIGN KEY (task_id) REFERENCES task(id) ON DELETE CASCADE,
    FOREIGN KEY (dictionary_id) REFERENCES dictionary(id) ON DELETE CASCADE
);

DO $$ BEGIN
    CREATE TYPE call_metrics_emotion_type AS ENUM ('neutral', 'positive', 'angry', 'sad', 'other');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS task_call_metrics (
    task_id UUID,
    call_duration real NOT NULL,
    time_to_answer real NOT NULL,
    total_employee_speech real NOT NULL,
    total_client_speech real NOT NULL,
    employee_client_speech_ratio real NOT NULL,
    employee_speech_ratio real NOT NULL,
    client_speech_ratio real NOT NULL,
    call_holds_count int NOT NULL,
    silence_pause_count int NOT NULL,
    total_employee_silence real NOT NULL,
    client_interruptions_count int NOT NULL,
    total_client_interruptions_duration real NOT NULL,
    avg_employee_words_per_min real NOT NULL,
    avg_client_words_per_min real NOT NULL,

    script_score int NOT NULL,
    employee_quality_score int NOT NULL,

    emotion_mode call_metrics_emotion_type,
    emotion_start_mode call_metrics_emotion_type,
    emotion_end_mode call_metrics_emotion_type,

    PRIMARY KEY (task_id),
    FOREIGN KEY (task_id) REFERENCES task(id) ON DELETE CASCADE
);

DO $$ BEGIN
    CREATE TYPE settings_type AS ENUM (
        'quality',
        'script'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS settings (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    project_id UUID NOT NULL,
    type settings_type NOT NULL,

    PRIMARY KEY (id)
);

DO $$ BEGIN
    CREATE TYPE settings_item_type AS ENUM (
        'speech_rate_ratio',
        'call_holds',
        'silence_pauses',
        'interruptions',
        'lacking_info_dict',
        'filler_words_dict',
        'slurred_speech_dict',
        'profanity_speech_dict',
        'dictionary'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS settings_item (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    settings_id uuid NOT NULL,
    settings_immutable bool DEFAULT false NOT NULL,
    type settings_item_type NOT NULL,
    name text NOT NULL,
    score_weight int NOT NULL,

    PRIMARY KEY (id), 
    FOREIGN KEY (settings_id) REFERENCES settings(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS settings_dict_item (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    settings_item_id uuid NOT NULL,
    dictionary_id SERIAL NOT NULL,
    contains bool DEFAULT true NOT NULL,

    PRIMARY KEY (id),
    FOREIGN KEY (settings_item_id) REFERENCES settings_item(id) ON DELETE CASCADE,
    FOREIGN KEY (dictionary_id) REFERENCES dictionary(id) ON DELETE CASCADE
);
