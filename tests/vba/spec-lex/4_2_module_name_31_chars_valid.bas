' INV: vbal.4.2
' SOURCE: spec ms-vbal 4.2 modules, a module name may be up to 31 characters long
' EXPECT: ok
'#MODULE Abcdefghij1234567890abcdefghijk standard
Public Sub Hello()
    Debug.Print "ok"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Abcdefghij1234567890abcdefghijk.Hello
End Sub
