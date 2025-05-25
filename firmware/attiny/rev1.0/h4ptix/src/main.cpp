#include <Wire.h>

#define I2C_SLAVE_ADDR 0x60
#define SDA_PIN        PIN_PA1
#define SCL_PIN        PIN_PA2

#define SHIFT_DATA_PIN   PIN_PA3
#define SHIFT_CLOCK_PIN  PIN_PA6
#define SHIFT_LATCH_PIN  PIN_PA7

#define NUM_PORTS  4

typedef struct  {
  bool active;
  uint32_t until;
  uint8_t outBit;
} port;

port ports[NUM_PORTS] = {
  {false, 0, 0x01},
  {false, 0, 0x02},
  {false, 0, 0x06},
  {false, 0, 0x07},
};

void writeShiftRegister(uint8_t value) {
  // MSB first, shift out
  for (int i = 7; i >= 0; i--) {
    digitalWrite(SHIFT_CLOCK_PIN, LOW);
    digitalWrite(SHIFT_DATA_PIN, (value >> i) & 0x01);
    digitalWrite(SHIFT_CLOCK_PIN, HIGH);
  }

  // Latch
  digitalWrite(SHIFT_LATCH_PIN, LOW);
  digitalWrite(SHIFT_LATCH_PIN, HIGH);
}

uint8_t calcRegister() {
  uint8_t out = 0;
  for (uint8_t i = 0; i < NUM_PORTS; i++) {
    if (ports[i].active) {
      out |= (1 << ports[i].outBit);
    }
  }

  return out;
}

void syncRegister() {
  writeShiftRegister(calcRegister());
}

void onReceive(int len) {
  uint8_t buf[3] = {0};

  if (len < sizeof(buf)) {
    return;
  }

  uint8_t bufIdx = 0;
  while (Wire.available() && bufIdx < sizeof(buf)) {
    buf[bufIdx++] = Wire.read();
    len--;
  }

  // read garbage if any
  while (Wire.available() && len > 0) {
    Wire.read();
    len--;
  }


  uint8_t port = buf[0];
  if (port >= NUM_PORTS) {
    return;
  }

  uint16_t duration = (uint16_t)buf[1] | ((uint16_t)buf[2] << 8);

  ports[port].active = true;
  ports[port].until = millis() + duration;
  syncRegister();
}

void onRequest() {
  // Echo back the last value
  Wire.write(calcRegister());
}

void setup() {
  // Shift register
  pinMode(SHIFT_DATA_PIN, OUTPUT);
  pinMode(SHIFT_CLOCK_PIN, OUTPUT);
  pinMode(SHIFT_LATCH_PIN, OUTPUT);

  syncRegister();

  // I2C
  Wire.pins(SDA_PIN, SCL_PIN);
  Wire.begin(I2C_SLAVE_ADDR);
  Wire.onReceive(onReceive);
  Wire.onRequest(onRequest);
}

void loop() {
  uint32_t now = millis();
  bool updated = false;

  for (int i = 0; i < NUM_PORTS; i++) {
    if (ports[i].active && now >= ports[i].until) {
      ports[i].active = false;
      updated = true;
    }
  }

  if (updated) {
    syncRegister();
  }
}
