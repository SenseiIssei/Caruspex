# Widok rentgenowski

**Widok rentgenowski** pokazuje podłączone auto jako szklany model, który można obracać. W środku widać silnik lub silniki elektryczne, skrzynię biegów, zbiornik lub akumulator wysokonapięciowy, układ wydechowy, chłodnicę, akumulator 12 V i gniazdo OBD, rozmieszczone według typu nadwozia i napędu rozpoznanego auta.

![Widok rentgenowski](screenshot-xray.png)

## Co się podświetla

- **Usterki** barwią swój obszar, a odpowiedzialny sterownik robi się czerwony.
- **Wartości na żywo** wiszą jako etykiety na części, do której należą: płyn chłodzący przy chłodnicy, obroty przy silniku, napięcie przy akumulatorze.
- **Sterowniki** znalezione podczas wyszukiwania siedzą tam, gdzie zwykle są, połączone liniami danych (CAN napędu, CAN komfortu, CAN diagnostyczny, K-Line). Gdy Caruspex odczytuje, małe pakiety wędrują do odpytywanego modułu.
- **Sprawdź auto** przenosi kamerę do każdego znaleziska, jednego po drugim.

Obracaj myszą lub palcem, przybliżaj kółkiem albo dwoma palcami; klawiatura też działa. Warstwy (nadwozie, napęd, chłodzenie, paliwo lub wysokie napięcie, elektryka, sieć) można wyłączać pojedynczo.

## Jak dokładny jest model?

Kształt pochodzi z prawdziwych wymiarów generacji, jeśli są znane (z Wikidata albo z danych technicznych producenta), a w przeciwnym razie z typowych wartości dla typu nadwozia i epoki. To nie jest wierna kopia konkretnego modelu: dane konstrukcyjne i schematy elektryczne producentów nie są ogólnie dostępne. Strona mówi, skąd pochodzi kształt.

Jeśli masz model 3D swojego auta w formacie glTF (`.glb` lub `.gltf`), możesz go wczytać przyciskiem **Wczytaj własny model 3D**; wnętrze zostaje z układu.

## Bez 3D

Na komputerach bez WebGL strona przechodzi na płaski widok z boku z tymi samymi informacjami.
