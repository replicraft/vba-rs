' INV: func.replace
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/replace-function
' EXPECT: xxx
' EXPECT: xAx
Public Sub Main()
    Debug.Print CStr(Replace("aAa", "a", "x", 1, -1, vbTextCompare))
    Debug.Print CStr(Replace("aAa", "a", "x", 1, -1, vbBinaryCompare))
End Sub
