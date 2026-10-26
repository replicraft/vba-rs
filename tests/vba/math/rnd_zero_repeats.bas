' INV: func.rnd
' SOURCE: spec ms-vbal 6.1.2.10.1.6 Rnd
' EXPECT: True
Public Sub Main()
    Dim a As Double
    a = Rnd
    Debug.Print Rnd(0) = a
End Sub
