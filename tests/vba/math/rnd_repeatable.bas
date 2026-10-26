' INV: func.rnd, stmt.randomize
' SOURCE: derived Rnd(-1) followed by Randomize n makes the sequence repeat (Rnd/Randomize pages)
' EXPECT: True
Public Sub Main()
    Dim a As Double, b As Double
    Rnd -1
    Randomize 7
    a = Rnd
    Rnd -1
    Randomize 7
    b = Rnd
    Debug.Print a = b
End Sub
