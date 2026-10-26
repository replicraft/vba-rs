' INV: vbal.6.1.2.10.1.6, func.rnd
' SOURCE: spec ms-vbal 6.1.2.10.1.6 Rnd(0) returns the most recently generated number
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim a As Single, b As Single
    a = Rnd
    b = Rnd(0)
    Debug.Print a = b
    a = Rnd(1)
    Debug.Print a = Rnd(0)
End Sub
