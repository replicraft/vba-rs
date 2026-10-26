' INV: stmt.option-base
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/option-base-statement
' EXPECT:  1  3 
' EXPECT:  1 
Option Base 1
Public Sub Main()
    Dim a(3) As Long
    Debug.Print LBound(a); UBound(a)
    Debug.Print LBound(Array(5, 6))
End Sub
