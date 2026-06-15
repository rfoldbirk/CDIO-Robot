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

## Control-Interface
Jeg vil gerne lave en hjemmeside, hvorfra man kan styre processen

Man skal kunne:
- Kalibrere farver, sådan at orange bliver den korrekte farve - udfra den farve man vælger fra billedet
- Når der scannes efter en farve, skal der være en usikkerhedsgrad man kan skrue op eller ned for via CI.
- Start og stop knap.
- Scanning foregår kontinuerligt.

## Ny Scanning
Alle pixels på billedet scannes for farven og bliver market. Pixels med en unøjagtig må også være inkluderet i resultatet, men det bliver noteret hvad deres distance er fra target farven.


Gruppér pixels sammen som er inden for en bestemt distance og find yder punkterne!
Hvis 2, 3 eller flere er grupperet sammen, så dividere vi med den højde vi ved en bordtennis bold bør være. Vi kan evt. kigge på en af dem, som 

For at finde diameteren på en bold, kan vi gå vores liste af bolde igennem og bruge den størrelse som optræder flest gange!


## HEX input istedet for RGB
Basically vil jeg gerne have at inputtet til analyzeren skal være
WHITE: hex, PRECISION: int, 
ORANGE: hex, PRECISION: int, 
RED: hex, PRECISION: int


## Ide til forbedring af scanning af boldene
Lige nu er det ikke hele bolden vi får med... Men det er måske fint!

Efter første scanning tjekker vi volumen af de scanninger og filtrer alle de små fra. Når vi har fundet de store, kan vi fortsætte søgningen!

Nu er det tid til at fortsætte søgningen af marks, men med en større accept for unøjagtighed. Vi ved allerede hvor vores bolde er, så vores søgning er kun der!

- [ ] Find de fire hjørner
- [ ] Find krydset


## Optimering!
- Efter første gennemgang og filtrering kan vi se inden for hvilke pixels vi faktisk behøver at scanne!
  Vi kan formentlig filtrerer en del pixels fra!
