' INV: vbal.3.3.5.3
' SOURCE: spec ms-vbal 3.3.5.3 special identifier forms, FOREIGN-NAME may spell a reserved word as a name
' EXPECT: 3
Public Sub Main()
    Dim [Loop] As Long
    [Loop] = 3
    Debug.Print CStr([Loop])
End Sub
