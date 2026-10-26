' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 at least one enum-element must be an enum-member (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Private Enum Bad
    Rem nothing here
End Enum
Public Sub Main()
    Debug.Print "ok"
End Sub
