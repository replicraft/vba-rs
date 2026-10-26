' INV: vbal.3.3.5.3
' SOURCE: spec ms-vbal 3.3.5.3 special identifier forms, FOREIGN-NAME [x] is used as an identifier whose name value is the bracketed text
' EXPECT: 5
' EXPECT: 4
' EXPECT: 9
Public Sub Main()
    Dim [my var] As Long
    [my var] = 5
    Debug.Print CStr([my var])
    Dim x As Long
    [X] = 4
    Debug.Print CStr(x)
    Dim [Mod] As Long
    [Mod] = 9
    Debug.Print CStr([mod])
End Sub
