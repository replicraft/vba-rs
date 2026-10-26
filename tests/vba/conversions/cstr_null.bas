' INV: func.cstr, lit.null
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' ERROR: 94
Public Sub Main()
    Debug.Print CStr(Null)
End Sub
