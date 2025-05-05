#include <Wire.h>

#define I2C_SLAVE_ADDRESS 0x60
#define SDA_PIN   PIN_PA1
#define SCL_PIN   PIN_PA2

#define DS_PIN    PIN_PA3  // Data
#define SHCP_PIN  PIN_PA6  // Shift Clock
#define STCP_PIN  PIN_PA7  // Latch Clock

#define PORT_CNT  4

volatile uint8_t shiftState = 0;
static uint32_t portTimers[PORT_CNT] = {0};
static bool portActive[PORT_CNT] = {false};
static uint8_t portBits[PORT_CNT] = {
  0x01,
  0x02,
  0x06,
  0x07,
};

uint8_t calcState() {
  uint8_t state = 0;
  for (uint8_t i = 0; i < PORT_CNT; i++) {
    if (portActive[i]) {
      state |= (1 << portBits[i]);
    }
  }

  return state;
}

void shiftOutState(uint8_t value) {
  // MSB first, shift out
  for (int i = 7; i >= 0; i--) {
    digitalWrite(SHCP_PIN, LOW);
    digitalWrite(DS_PIN, (value >> i) & 0x01);
    digitalWrite(SHCP_PIN, HIGH);
  }

  // Latch
  digitalWrite(STCP_PIN, LOW);
  digitalWrite(STCP_PIN, HIGH);
}

void shiftStateUpdate() {
  shiftState = calcState();
  shiftOutState(shiftState);
}

void onReceive(int len) {
  if (len < 3) {
    return;
  }

  uint8_t port = Wire.read();
  if (port >= PORT_CNT) {
    return;
  }

  uint16_t duration = Wire.read();  // low byte
  duration |= (Wire.read() << 8);  // high byte

  portActive[port] = true;
  portTimers[port] = millis() + duration;

  shiftStateUpdate();
}

void onRequest() {
  Wire.write(calcState()); // Echo back the last value
}

void setup() {
  // Shift register
  pinMode(DS_PIN, OUTPUT);
  pinMode(SHCP_PIN, OUTPUT);
  pinMode(STCP_PIN, OUTPUT);

  shiftOutState(0x00);

  // I2C
  Wire.pins(SDA_PIN, SCL_PIN);
  Wire.begin(I2C_SLAVE_ADDRESS);
  Wire.onReceive(onReceive);
  Wire.onRequest(onRequest);
}

void loop() {
  uint32_t now = millis();
  bool updated = false;

  for (int i = 0; i < PORT_CNT; i++) {
    if (portActive[i] && now >= portTimers[i]) {
      portActive[i] = false;
      updated = true;
    }
  }

  if (updated) {
    shiftStateUpdate();
  }
}
