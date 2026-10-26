' INV: stmt.randomize, func.rnd
' SOURCE: spec ms-vbal 6.1.2.10.1.6 Rnd result range
' EXPECT: True
Public Sub Main()
    Randomize
    Dim x As Double
    x = Rnd
    Debug.Print x >= 0 And x < 1
End Sub
