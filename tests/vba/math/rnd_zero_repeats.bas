' INV: func.rnd
' SOURCE: derived Rnd(0) returns the most recently generated number
' EXPECT: True
Public Sub Main()
    Dim a As Double
    a = Rnd
    Debug.Print Rnd(0) = a
End Sub
