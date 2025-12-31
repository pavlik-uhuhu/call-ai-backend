INSERT INTO dictionary (name, participant) VALUES ('lacking_info', 'employee');
INSERT INTO dictionary (name, participant) VALUES ('slurred_speech', 'client');
INSERT INTO dictionary (name, participant) VALUES ('profanity_speech', 'employee');
INSERT INTO dictionary (name, participant) VALUES ('banned_words', 'employee');
INSERT INTO dictionary (name, participant) VALUES ('filler_words', 'employee');
INSERT INTO dictionary (name, participant) VALUES ('welcome_words', 'employee');
INSERT INTO dictionary (name, participant) VALUES ('introduction_words', 'employee');
INSERT INTO dictionary (name, participant) VALUES ('presentation_words', 'employee');
INSERT INTO dictionary (name, participant) VALUES ('farewell_words', 'employee');

INSERT INTO phrase (dictionary_id, text) 
VALUES 
    (1, 'не могу ответить'),
    (1, 'не могу помочь'),
    (1, 'не могу сказать'),
    (1, 'сложно сказать'),
    (1, 'не знаю'),
    (1, 'не помогу'),
    (1, 'не подскажу'),
    (1, 'сложно сказать'),
    (1, 'не уверен'),
    (1, 'не владею'),
    (1, 'не обладаю'),
    (1, 'нужно уточнить'),
    (1, 'необходимо уточнить'),
    (1, 'нужно проверить'),
    (1, 'необходимо проверить');

INSERT INTO phrase (dictionary_id, text) 
VALUES 
    (2, 'громче можно'),
    (2, 'чуть громче'),
    (2, 'не расслышал'),
    (2, 'громче говорите'),
    (2, 'можете громче'),
    (2, 'не услышал'),
    (2, 'помедленнее говорите'),
    (2, 'что вы говорите'),
    (2, 'не пойму вас'),
    (2, 'громче чуть'),
    (2, 'можно громче'),
    (2, 'помедленнее пожалуйста'),
    (2, 'пожалуйста помедленнее'),
    (2, 'не понимаю вас'),
    (2, 'говорите громче'),
    (2, 'не понимаю что вы говорите'),
    (2, 'не пойму что вы говорите'),
    (2, 'громче можете'),
    (2, 'мычите под нос'),
    (2, 'вы мямлите'),
    (2, 'говорите медленнее'),
    (2, 'невнятно говорите'),
    (2, 'повторите еще раз'),
    (2, 'повторите медленнее'),
    (2, 'не разобрал'),
    (2, 'я вас не понял'),
    (2, 'что вы сказали'),
    (2, 'можете говорить чётче'),
    (2, 'сложно вас понять'),
    (2, 'говорите слишком быстро');

INSERT INTO phrase (dictionary_id, text) 
VALUES 
    (3, 'солнышко мое вставай'),
    (3, 'ласковый и такой красивый');

INSERT INTO phrase (dictionary_id, text) 
VALUES 
    (4, 'секундочку'),
    (4, 'заказик'),
    (4, 'минутку'),
    (4, 'минуточку'),
    (4, 'ладненько');

INSERT INTO phrase (dictionary_id, text) 
VALUES 
    (5, 'на самом деле'),
    (5, 'так скажем'),
    (5, 'так сказать'),
    (5, 'блин'),
    (5, 'в общем то'),
    (5, 'вообще то'),
    (5, 'как бы'),
    (5, 'короче'),
    (5, 'в общем то'),
    (5, 'на самом деле'),
    (5, 'честно говоря'),
    (5, 'грубо говоря'),
    (5, 'скажем так'),
    (5, 'типа'),
    (5, 'ну то есть'),
    (5, 'ой ой ой'),
    (5, 'это самое'),
    (5, 'как его там'),
    (5, 'ой я'),
    (5, 'все такое'),
    (5, 'дело в том что'),
    (5, 'собственно'),
    (5, 'типа того'),
    (5, 'жесть'),
    (5, 'и так далее'),
    (5, 'ой ну'),
    (5, 'собственно говоря'),
    (5, 'походу'),
    (5, 'как сказать'),
    (5, 'в натуре'),
    (5, 'как говорится'),
    (5, 'ой извините'),
    (5, 'ой это'),
    (5, 'это самое'),
    (5, 'короче говоря'),
    (5, 'типа того');

INSERT INTO phrase (dictionary_id, text) 
VALUES 
    (6, 'здравствуйте'),
    (6, 'добрый день'),
    (6, 'добрый вечер'),
    (6, 'доброе утро'),
    (6, 'приветствую');

INSERT INTO phrase (dictionary_id, text) 
VALUES (7, 'меня зовут');

INSERT INTO phrase (dictionary_id, text) 
VALUES (8, 'компания безлимит');

INSERT INTO phrase (dictionary_id, text) 
VALUES 
    (9, 'всего доброго'),
    (9, 'всего хорошего'),
    (9, 'хорошего дня'),
    (9, 'хорошего вечера'),
    (9, 'до свидания'),
    (9, 'до скорого'),
    (9, 'до скорой');
