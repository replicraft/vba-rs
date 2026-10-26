' INV: stmt.option-base, stmt.redim
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/option-base-statement
' EXPECT:  1  4 
Option Base 1
Public Sub Main()
    Dim a() As Long
    ReDim a(4)
    Debug.Print LBound(a); UBound(a)
End Sub
