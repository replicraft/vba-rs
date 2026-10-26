' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 ByVal parameter may not carry an array designator (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Sub S(ByVal a() As Long)
End Sub
Public Sub Main()
    Debug.Print "ok"
End Sub
