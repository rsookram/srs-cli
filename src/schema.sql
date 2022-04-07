CREATE TABLE Deck(
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    creationTimestamp INTEGER NOT NULL, -- In milliseconds since Unix epoch
    intervalModifier INTEGER NOT NULL DEFAULT 100
);

CREATE TABLE Card(
    id INTEGER PRIMARY KEY,
    deckId INTEGER NOT NULL REFERENCES Deck(id) ON DELETE CASCADE,
    front TEXT NOT NULL,
    back TEXT NOT NULL,
    creationTimestamp INTEGER NOT NULL -- In milliseconds since Unix epoch
);

CREATE INDEX cardCreationTimestamp ON Card(creationTimestamp);
CREATE INDEX cardDeckId ON Card(deckId);

CREATE TABLE Answer(
    cardId INTEGER NOT NULL REFERENCES Card(id) ON DELETE CASCADE,
    isCorrect INTEGER NOT NULL,
    timestamp INTEGER NOT NULL -- In milliseconds since Unix epoch
);

CREATE INDEX answerCardId ON Answer(cardId);
CREATE INDEX answerTimestamp ON Answer(timestamp);

CREATE TABLE Schedule(
    cardId INTEGER PRIMARY KEY REFERENCES Card(id) ON DELETE CASCADE,
    scheduledForTimestamp INTEGER, -- In milliseconds since Unix epoch, NULL when suspended
    intervalDays INTEGER, -- 0 for new cards, NULL when suspended
    isLeech INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX scheduleScheduledForTimestamp ON Schedule(scheduledForTimestamp);
CREATE INDEX scheduleIsLeech ON Schedule(isLeech);
