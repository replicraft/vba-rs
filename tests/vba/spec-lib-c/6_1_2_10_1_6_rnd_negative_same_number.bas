' INV: vbal.6.1.2.10.1.6, func.rnd
' SOURCE: spec ms-vbal 6.1.2.10.1.6 Rnd with a negative Number gives the same number every time for that seed
' EXPECT: same=True
' EXPECT: inrange=True
Public Sub Main()
    Dim a As Single, b As Single
    a = Rnd(-5)
    Randomize 12
    b = Rnd(-5)
    Debug.Print "same=" & (a = b)
    Debug.Print "inrange=" & (a >= 0 And a < 1)
End Sub
