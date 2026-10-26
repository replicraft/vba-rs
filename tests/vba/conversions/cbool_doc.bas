' INV: func.cbool
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(CBool(5 = 5))
    Debug.Print CStr(CBool(0))
End Sub
