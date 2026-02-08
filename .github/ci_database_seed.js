function mulberry32(a) {
    return function () {
        let t = a += 0x6D2B79F5;
        t = Math.imul(t ^ t >>> 15, t | 1);
        t ^= t + Math.imul(t ^ t >>> 7, t | 61);
        return ((t ^ t >>> 14) >>> 0) / 4294967296;
    }
}

const nextRand = mulberry32(12345);

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
const baseDate = new Date('2026-01-01T00:00:00Z');
for (let i = 0; i < batchSize; i++) {
    telemetryDocs.push({
        timestamp: new Date(baseDate.getTime() + (i * 1000)),
        sensorId: `sensor_${Math.floor(nextRand() * 100)}`,
        value: nextRand() * 100
    });
}
db.telemetry.insertMany(telemetryDocs);

const userDocs = [];
for (let i = 0; i < batchSize; i++) {
    userDocs.push({
        email: `user_${i}@example.com`,
        status: nextRand() > 0.5 ? "active" : "inactive",
        metadata: {
            loginCount: Math.floor(nextRand() * 500),
            lastSeen: new Date(baseDate.getTime() - (nextRand() * 1000000))
        }
    });
}
db.users.insertMany(userDocs);