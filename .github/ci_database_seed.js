const dbName = 'example';
const db = db.getSiblingDB(dbName);

db.createCollection("telemetry", {
    timeseries: {
        timeField: "timestamp",
        metaField: "sensorId",
        granularity: "seconds"
    }
});

db.createCollection("users", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            required: ["email", "status"],
            properties: {
                email: {bsonType: "string", pattern: "@"},
                status: {enum: ["active", "inactive"]}
            }
        }
    }
});
db.users.createIndex({email: 1}, {unique: true});

const batchSize = 10000;

const telemetryDocs = [];
for (let i = 0; i < batchSize; i++) {
    telemetryDocs.push({
        timestamp: new Date('2026-01-01T00:00:00Z'),
        sensorId: `sensor_${i}`,
        value: i * 100
    });
}
db.telemetry.insertMany(telemetryDocs);

const userDocs = [];
for (let i = 0; i < batchSize; i++) {
    userDocs.push({
        email: `user_${i}@example.com`,
        status: (i / 2) !== 0 ? "active" : "inactive",
        metadata: {
            loginCount: i,
            lastSeen: new Date('2026-01-01T00:00:00Z')
        }
    });
}
db.users.insertMany(userDocs);