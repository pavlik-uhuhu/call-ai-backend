DO $$ BEGIN
    CREATE TYPE participant_type AS ENUM ('client', 'employee');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS dictionary (
    id SERIAL,
    name text NOT NULL,
    participant participant_type NOT NULL,

    PRIMARY KEY (id)
 );

CREATE TABLE IF NOT EXISTS phrase (
    id BIGSERIAL,
    dictionary_id int NOT NULL,
    text text NOT NULL,

    PRIMARY KEY (id),
    FOREIGN KEY (dictionary_id) REFERENCES dictionary(id) ON DELETE CASCADE
);
