' INV: stmt.for-each-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/for-eachnext-statement
' EXPECT:  10 
' EXPECT:  20 
' EXPECT:  30 
Public Sub Main()
    Dim a As Variant, v As Variant
    a = Array(10, 20, 30)
    For Each v In a
        Debug.Print v
    Next
End Sub
