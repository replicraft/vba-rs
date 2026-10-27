' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 optional parameter typed only by a Def directive needs a default value (spec-strict: a warning by default, the program runs)
' EXPECT: ok
DefLng A-Z
Sub S(Optional a)
End Sub
Public Sub Main()
    Debug.Print "ok"
End Sub
