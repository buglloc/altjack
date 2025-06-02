#include <Wire.h>

#define I2C_SLAVE_ADDR 0x60
#define SDA_PIN        PIN_PA1
#define SCL_PIN        PIN_PA2

#define NUM_PORTS  4

typedef struct  {
  bool active;
  uint32_t until;
  uint8_t pin;
} port;

port ports[NUM_PORTS] = {
  {false, 0, PIN_PC3},
  {false, 0, PIN_PC2},
  {false, 0, PIN_PC1},
  {false, 0, PIN_PC0},
};

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
  digitalWrite(ports[port].pin, HIGH);
}

void onRequest() {
  // Echo back the last value
  uint8_t out = 0;
  for (uint8_t i = 0; i < NUM_PORTS; i++) {
    if (ports[i].active) {
      out |= (1 << i);
    }
  }
  Wire.write(out);
}

void setup() {
  // GPIO
  for (int i = 0; i < NUM_PORTS; i++) {
    pinMode(ports[i].pin, OUTPUT);
    digitalWrite(ports[i].pin, LOW);
  }

  // I2C
  Wire.pins(SDA_PIN, SCL_PIN);
  Wire.begin(I2C_SLAVE_ADDR);
  Wire.onReceive(onReceive);
  Wire.onRequest(onRequest);
}

void loop() {
  uint32_t now = millis();

  for (int i = 0; i < NUM_PORTS; i++) {
    if (ports[i].active && now >= ports[i].until) {
      ports[i].active = false;
      digitalWrite(ports[i].pin, LOW);
    }
  }
}
