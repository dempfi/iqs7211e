/******************************************************************************
 *                                                                            *
 *                                Copyright by                                *
 *                                                                            *
 *                              Azoteq (Pty) Ltd                              *
 *                          Republic of South Africa                          *
 *                                                                            *
 *                           Tel: +27(0)21 863 0033                           *
 *                           E-mail: info@azoteq.com                          *
 *                                                                            *
 * ========================================================================== *
 * @file        iqs7211e-example-code.ino                                     *
 * @brief       IQS7211E Mini Trackpad EV-Kit Example code                    *
 *              (PCB Version - AZP1190B1)                                     *
 *              This example demonstrates how to write the desired settings   *
 *              to the IQS7211E in order to use the IQS7211E Mini Trackpad    *
 *              EV-Kit.                                                       *
 *                                                                            *
 *              All data is displayed over serial communication with the      *
 *              following outputs:                                            *
 *                  - Gestures output(if enabled)                             *
 *                  - Finger 1: X and Y Coordinates                           *
 *                  - Power Mode Feedback                                     *
 *                  - Forced Communication                                    *
 * @author      Azoteq PTY Ltd                                                *
 * @version     v1.1                                                          *
 * @date        2023-07-14                                                    *
 ******************************************************************************/

#include <Arduino.h>
#include "src\IQS7211E\IQS7211E.h"

/*** Defines ***/
#define DEMO_IQS7211E_ADDR        0x56
#define DEMO_IQS7211E_POWER_PIN   4
#define DEMO_IQS7211E_RDY_PIN     7

/*** Instances ***/
IQS7211E iqs7211e;

/*** Global Variables ***/
bool show_data = false;
iqs7211e_power_modes running_power_mode = IQS7211E_IDLE;
uint16_t running_x_output = 65535;
uint16_t running_y_output = 65535;
iqs7211e_gestures_e running_gestures = IQS7211E_GESTURE_NONE;

void setup()
{
  /* Start Serial Communication */
  Serial.begin(115200);
  while(!Serial);
  Serial.println("Start Serial communication");

  /* Power On IQS7211E */
  pinMode(DEMO_IQS7211E_POWER_PIN, OUTPUT);
  delay(200);
  digitalWrite(DEMO_IQS7211E_POWER_PIN, LOW);
  delay(200);
  digitalWrite(DEMO_IQS7211E_POWER_PIN, HIGH);

  /* Initialize the IQS7211E with input parameters device address and RDY pin */
  iqs7211e.begin(DEMO_IQS7211E_ADDR, DEMO_IQS7211E_RDY_PIN);
  Serial.println("IQS7211E Ready");
  delay(1);
}

void loop()
{
  /* Read new data from IQS7211E if available  (RDY Line Low) */
  iqs7211e.run();

  /* Function to initialize a force communication window. */
  force_comms_and_reset();

  /* If data was updated, display data read from IQS7211E */
  if (iqs7211e.new_data_available)
  {
   /* Print the following if the new data did not come from a force comms command */
    if (!show_data)
    {
      if(printData())
      {
        printGesture();
        printCoordinates();
        printPowerMode();
      }
    }
    /* Display the heading when a finger is lifted from the trackpad */
    printHeading();
    /* Set this flag to false to indicate that the new data was already displayed/used */
    iqs7211e.new_data_available = false;
    /* Check if a force comms command was sent and if we should display the data read */
    show_iqs7211e_data();
  }
}

/* Check if one of the gesture flags is set and display which one */
void printGesture(void)
{
  iqs7211e_gestures_e buffer =  iqs7211e.get_touchpad_event();

  switch (buffer)
  {
    case IQS7211E_GESTURE_SINGLE_TAP:
      Serial.print("Single Tap\t\t");
    break;

    case IQS7211E_GESTURE_DOUBLE_TAP:
      Serial.print("Double Tap\t\t");
    break;

    case IQS7211E_GESTURE_TRIPLE_TAP:
      Serial.print("Triple Tap\t\t");
    break;

    case IQS7211E_GESTURE_PRESS_HOLD:
      Serial.print("Press and Hold\t\t");
    break;

    case IQS7211E_GESTURE_PALM_GESTURE:
      Serial.print("Palm Gesture\t\t");
    break;

    case IQS7211E_GESTURE_SWIPE_X_POSITIVE:
      Serial.print("Swipe X +\t\t");
    break;

    case IQS7211E_GESTURE_SWIPE_X_NEGATIVE:
      Serial.print("Swipe X -\t\t");
    break;

    case IQS7211E_GESTURE_SWIPE_Y_POSITIVE:
      Serial.print("Swipe Y +\t\t");
    break;

    case IQS7211E_GESTURE_SWIPE_Y_NEGATIVE:
      Serial.print("Swipe Y -\t\t");
    break;

    case IQS7211E_GESTURE_SWIPE_HOLD_X_POSITIVE:
      Serial.print("Swipe and Hold X +\t");
    break;

    case IQS7211E_GESTURE_SWIPE_HOLD_X_NEGATIVE:
      Serial.print("Swipe and Hold X -\t");
    break;

    case IQS7211E_GESTURE_SWIPE_HOLD_Y_POSITIVE:
      Serial.print("Swipe and Hold Y +\t");
    break;

    case IQS7211E_GESTURE_SWIPE_HOLD_Y_NEGATIVE:
      Serial.print("Swipe and Hold Y -\t");
    break;

    default:
      Serial.print("-\t\t\t");
    break;
  }
  
  /* Update the running gesture value with the value from the buffer */
  running_gestures = buffer;
}

/* Check Power mode and print out the current power mode */
void printPowerMode(void)
{
  iqs7211e_power_modes buffer = iqs7211e.getPowerMode();

  switch (buffer)
  {
    case IQS7211E_ACTIVE:
      Serial.println("Active Mode");
      break;

    case IQS7211E_IDLE_TOUCH:
      Serial.println("Idle-Touch Mode");
      break;

    case IQS7211E_IDLE:
      Serial.println("Idle Mode");
      break;

    case IQS7211E_LP1:
      Serial.println("Low Power 1 Mode");
      break;

    case IQS7211E_LP2:
      Serial.println("Low Power 2 Mode");
      break;

    default:
      Serial.println("Unknown Mode");
      break;
    }

    /* Update the running power mode value with the buffer value */
    running_power_mode = buffer;
}

/* Function to print heading for the Serial data display demo purposes */
void printHeading(void)
{
  /* Check if it is necessary to display the heading */
  if(
         (iqs7211e.getAbsXCoordinate(FINGER_1)  == 65535)
      && (iqs7211e.getAbsYCoordinate(FINGER_1)  == 65535)
      && (iqs7211e.IQSMemoryMap.GESTURES[0]     == 0    )
      && (iqs7211e.IQSMemoryMap.GESTURES[1]     == 0    )
    )
  {
    Serial.println("\nGesture:\t\tFinger 1 X:\tFinger 1 Y:\tPower Mode:");
  }
}

/* Function to determine if it is necessary to print all the relevant data 
in the serial terminal */
bool printData(void)
{
/* See if it is necessary to display the button state, power mode or gestures */
  if(
      (
        (iqs7211e.getPowerMode()              != running_power_mode)
      ||(iqs7211e.get_touchpad_event()        != running_gestures)
      ||(iqs7211e.getAbsXCoordinate(FINGER_1) != running_x_output)
      ||(iqs7211e.getAbsYCoordinate(FINGER_1) != running_y_output)
      )
    )
  {
    return true; // Let the main loop know to display the latest values
  }
  else
  {
    return false; // Let the main loop know to not display the latest values
  }

  return false;
}

/* Function to print the X and Y coordinates of finger 1 */
void printCoordinates(void)
{
  uint16_t buffer = iqs7211e.getAbsXCoordinate(FINGER_1);

  Serial.print(buffer);       // Print X coordinates
  running_x_output = buffer; 
  Serial.print("\t\t");       // Spacing to match headings in Print

  buffer = iqs7211e.getAbsYCoordinate(FINGER_1);

  Serial.print(buffer);       // Print Y coordinates
  running_y_output = buffer;
  Serial.print("\t\t");       // Spacing to match headings in Print
}

/* Force the IQS7211E to open a RDY window and read the current state of the device */
void force_comms_and_reset(void)
{
  char message = read_message();

  /* If an 'f' was received over serial, open a forced communication window and
  print the new data received */
  if(message == 'f')
  {
    iqs7211e.force_I2C_communication(); // prompt the IQS7211E
    show_data = true;
  }

  /* If an 'r' was received over serial, request a software(SW) reset */
  if(message == 'r')
  {
    Serial.println("Software Reset Requested!");
    iqs7211e.force_I2C_communication(); // Request a RDY window
    iqs7211e.iqs7211e_state.state = IQS7211E_STATE_SW_RESET;
    running_power_mode = IQS7211E_IDLE;
    running_x_output = 65535;
    running_y_output = 65535;
    running_gestures = IQS7211E_GESTURE_NONE;
  }
}

/* Read message sent over serial communication */
char read_message(void)
{
  while (Serial.available())
  {
    if (Serial.available() > 0)
    {
      return Serial.read();
    }
  }

  return '\n';
}

/* Shows the current IQS7211E data when a force comms command was sent */
void show_iqs7211e_data()
{
  if (show_data)
  {
    Serial.println("\n**************************************************************************");
    Serial.println("********************* IQS7211E DATA - FORCED COMMS ***********************");
    Serial.println("**************************************************************************");
    Serial.println("Gesture:\t\tFinger 1 X:\tFinger 1 Y:\tPower Mode:");
    printGesture();
    printCoordinates();
    printPowerMode();
    Serial.println("**************************************************************************");
    show_data = false;
  }
}
