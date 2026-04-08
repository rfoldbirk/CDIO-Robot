# Game Plan
- Tag billede
- Scan billede
- Lav rute
- Følg rute

# Idéer

## Boldens centrum
I forhold til at finde boldens centrum, så kan vi, for hver markør måle afstanden fra centrum ud til fire punkter. På den måde burde man med én måling kunne finde centrum!

Der opstår muligvis problemer med bolde tæt på hinanden.

## Auto kalibrering
Derudover, i forhold til auto kalibrering, så vi sikkert lave et gætte system, hvis programmet på forhånd ved hvor mange bolde der skal være. Hvis antallet af bolde ikke går op, kan den starte forfra med et nyt gæt. Hvert gæt skal selvfølgelig ikke være tilfældigt, men ud fra hvordan hvid eller orange bør være. Orange bliver formentlig sværest af kalibrere.



## Scanning
Tegn på billedet en cirkel på 32 pixels, for at se hvor meget det er.
Scan billedet i boldens længde til at starte med. Vi tager gennemsnittet, og hvis gennemsnittet er orange nok, så tilføjer vi feltet som en potentiel bold.
