' INV: func.ubound
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/ubound-function
' ERROR: 9
Public Sub Main()
    Dim a(3) As Long
    Debug.Print UBound(a, 2)
End Sub
