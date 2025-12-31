DO $$ 
DECLARE 
    quality_settings_id uuid;
    script_settings_id uuid;

    lacking_info_item_id uuid;
    filler_words_item_id uuid;
    slurred_speech_item_id uuid;
    profanity_speech_item_id uuid;

    welcome_words_item_id uuid;
    introduction_words_item_id uuid;
    presentation_words_item_id uuid;
    farewell_words_item_id uuid;
BEGIN
    INSERT INTO settings (project_id, type) VALUES ('00000000-0000-0000-0000-000000000000', 'quality')
    RETURNING id INTO quality_settings_id;

    INSERT INTO settings (project_id, type) VALUES ('00000000-0000-0000-0000-000000000000', 'script')
    RETURNING id INTO script_settings_id;

    INSERT INTO settings_item (settings_id, settings_immutable, type, name, score_weight) 
    VALUES 
        (quality_settings_id, true, 'speech_rate_ratio', 'Соответствие темпа', 5),
        (quality_settings_id, true, 'call_holds', 'Отсутствие удержаний звонка', 15),
        (quality_settings_id, true, 'silence_pauses', 'Отсутствие пауз', 10),
        (quality_settings_id, true, 'interruptions', 'Отсутствие перебиваний', 15);

    INSERT INTO settings_item (settings_id, settings_immutable, type, name, score_weight) VALUES (
        quality_settings_id, 
        true,
        'lacking_info_dict',
        'Знание о продукте',
        15
    ) RETURNING id INTO lacking_info_item_id;
    INSERT INTO settings_item (settings_id, settings_immutable, type, name, score_weight) VALUES (
        quality_settings_id, 
        true,
        'filler_words_dict',
        'Чистота речи',
        10
    ) RETURNING id INTO filler_words_item_id;
    INSERT INTO settings_item (settings_id, settings_immutable, type, name, score_weight) VALUES (
        quality_settings_id, 
        true,
        'slurred_speech_dict',
        'Внятность речи',
        15
    ) RETURNING id INTO slurred_speech_item_id;
    INSERT INTO settings_item (settings_id, settings_immutable, type, name, score_weight) VALUES (
        quality_settings_id, 
        true,
        'profanity_speech_dict',
        'Отсутствие запрещенных слов',
        15
    ) RETURNING id INTO profanity_speech_item_id;

    INSERT INTO settings_dict_item (settings_item_id, dictionary_id, contains) 
    VALUES 
        (lacking_info_item_id, 1, false),
        (filler_words_item_id, 5, false),
        (slurred_speech_item_id, 2, false),
        (profanity_speech_item_id, 3, false);

    INSERT INTO settings_item (settings_id, settings_immutable, type, name, score_weight) VALUES (
        script_settings_id, 
        false,
        'dictionary',
        'Приветствие',
        25
    ) RETURNING id INTO welcome_words_item_id;
    INSERT INTO settings_item (settings_id, settings_immutable, type, name, score_weight) VALUES (
        script_settings_id, 
        false,
        'dictionary',
        'Представление исполнителя',
        25
    ) RETURNING id INTO introduction_words_item_id;
    INSERT INTO settings_item (settings_id, settings_immutable, type, name, score_weight) VALUES (
        script_settings_id, 
        false,
        'dictionary',
        'Представление компании',
        25
    ) RETURNING id INTO presentation_words_item_id;
    INSERT INTO settings_item (settings_id, settings_immutable, type, name, score_weight) VALUES (
        script_settings_id, 
        false,
        'dictionary',
        'Прощание исполнителя',
        25
    ) RETURNING id INTO farewell_words_item_id;

    INSERT INTO settings_dict_item (settings_item_id, dictionary_id, contains) 
    VALUES 
        (welcome_words_item_id, 6, true),
        (introduction_words_item_id, 7, true),
        (presentation_words_item_id, 8, true),
        (farewell_words_item_id, 9, true);
END $$;
