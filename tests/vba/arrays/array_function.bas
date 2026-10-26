' INV: func.array
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/array-function
' EXPECT:  0  2 
' EXPECT: a 2  3.5 
Public Sub Main()
    Dim v As Variant
    v = Array("a", 2, 3.5)
    Debug.Print LBound(v); UBound(v)
    Debug.Print v(0); v(1); v(2)
End Sub
