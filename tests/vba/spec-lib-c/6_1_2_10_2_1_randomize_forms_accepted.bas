' INV: vbal.6.1.2.10.2.1, stmt.randomize
' SOURCE: spec ms-vbal 6.1.2.10.2.1 Randomize accepts a missing, Empty or numeric Number
' EXPECT: ok
Public Sub Main()
    Dim e As Variant, x As Single
    Randomize
    Randomize e
    Randomize 3
    Randomize 2.5
    x = Rnd
    If x >= 0 And x < 1 Then Debug.Print "ok" Else Debug.Print "bad"
End Sub
