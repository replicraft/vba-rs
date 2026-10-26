' INV: func.mid
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/mid-function
' ERROR: 5
Public Sub Main()
    Debug.Print Mid("abc", 1, -1)
End Sub
