' INV: func.replace, lit.null
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/replace-function
' ERROR: 94
Public Sub Main()
    Debug.Print Replace(Null, "a", "b")
End Sub
