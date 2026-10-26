' INV: stmt.for-each-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/for-eachnext-statement For Each over an object that exposes no enumerator is Object doesn't support this property or method (438)
' ERROR: 438
'#MODULE Plain class
Public Sub Hello()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim p As New Plain
    Dim x As Variant
    For Each x In p
        Debug.Print x
    Next
End Sub
