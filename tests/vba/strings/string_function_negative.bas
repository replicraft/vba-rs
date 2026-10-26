' INV: func.string
' SOURCE: spec ms-vbal 6.1.2.11.1.38 String function uses the first character of a string argument
' EXPECT: 5
Public Sub Main()
    Debug.Print CStr(Len(String(5, "ab")))
End Sub
