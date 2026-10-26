' INV: func.array, func.ubound
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/array-function
' EXPECT: -1 
Public Sub Main()
    Dim v As Variant
    v = Array()
    Debug.Print UBound(v)
End Sub
