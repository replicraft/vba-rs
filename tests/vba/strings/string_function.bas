' INV: func.string
' SOURCE: spec ms-vbal 6.1.2.11.1.38 String function
' EXPECT: xxx
' EXPECT: aaa
' EXPECT: AA
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(String(3, "x"))
    Debug.Print CStr(String(3, "abc"))
    Debug.Print CStr(String(2, 65))
    Debug.Print CStr(String(0, "x"))
End Sub
