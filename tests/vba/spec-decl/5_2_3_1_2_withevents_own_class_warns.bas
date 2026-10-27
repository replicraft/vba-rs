' INV: vbal.5.2.3.1.2, decl.withevents
' SOURCE: spec ms-vbal 5.2.3.1.2 WithEvents type must not be the containing class (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Public Sub Main()
    Debug.Print "ok"
End Sub
'#MODULE Selfish class
Public Event Fired()
Private WithEvents me2 As Selfish
