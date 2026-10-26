' INV: func.rnd, stmt.randomize
' SOURCE: spec ms-vbal 6.1.2.10.1.6 Rnd
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
