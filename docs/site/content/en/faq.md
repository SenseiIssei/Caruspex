# Frequently asked questions

## Does Caruspex cost anything?

No. Caruspex is free software under the MIT licence, without adverts, accounts or paid extras.

## Which adapter should I buy?

For most people a USB ELM327 with an FTDI or CH340 chip, or an OBDLink (Bluetooth or USB), is the best start. For VW, Audi, Seat and Skoda before about 2004 add a [KKL cable](kkl.md); for BMW and Mini from the F series an [ENET cable](enet.md). The [overview of interfaces](interfaces.md) compares them.

## Will it work with my car?

Any car with OBD-II or EOBD works for the engine: in Europe petrol cars from 2001 and diesels from 2004, in the US from 1996. Older VW group cars from about 1990 work over the KKL cable. The **Vehicles** page tells you which way works for a given car. Motorcycles from Euro 5 work with an adapter cable for their diagnostic plug.

## Has it been tested on my car?

Honestly: not yet. Caruspex is tested against simulated control units; on real hardware only an OBDLink with the STN2232 chip has been checked so far, without a car. Please help with a [test report](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Can Caruspex break my car?

Caruspex mostly reads. It writes only after asking you: clearing fault codes and, over the VAG KKL cable, the output test and the throttle basic setting. It never codes, never flashes and never changes adaptation values. Do not use it while driving.

## Can it change the mileage, the immobiliser or code features?

No, and it will not. That is legally sensitive and not the purpose of this tool.

## Why does the adapter say version 2.1 but some things fail?

Many cheap adapters report a newer ELM327 version than they really are. The **Adapter test** finds out which commands the adapter really knows.

## Why are values slow on my older car?

Cars on the K-Line (about 2001 to 2007 in Europe) answer slowly by design. CAN cars are much faster.

## Does Caruspex send my data anywhere?

No. Only the update check asks GitHub for the newest version, and only after you said yes to it; you can change that any time. See [privacy](privacy.md).

## How can I help?

Send a test report from your car, share a recording, report bugs, or improve a translation: the guide is [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md), and the button **Improve translation** in the settings opens it.

Code is welcome too: [CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md) explains how to build Caruspex and what a pull request needs.
