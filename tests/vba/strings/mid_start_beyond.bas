' INV: func.mid
' SOURCE: spec ms-vbal 6.1.2.11.1.25 Mid start greater than length gives empty string
' EXPECT: 
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(Mid("abc", 4))
    Debug.Print CStr(Mid("abc", 10, 2))
End Sub
