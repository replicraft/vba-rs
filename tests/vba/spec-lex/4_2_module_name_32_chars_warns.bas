' INV: vbal.4.2
' SOURCE: spec ms-vbal 4.2 modules, the maximum length of a module name is 31 characters (spec-strict: a warning by default, the program runs)
' EXPECT: ok
'#MODULE Abcdefghij1234567890abcdefghijkl standard
Public Sub Hello()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print "ok"
End Sub
