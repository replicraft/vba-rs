' INV: expr.new, stmt.sub, stmt.function
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/function-statement
' EXPECT:  2 
'#MODULE Counter class
Private n As Long
Public Sub Inc()
    n = n + 1
End Sub
Public Function Value() As Long
    Value = n
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New Counter
    c.Inc: c.Inc
    Debug.Print c.Value()
End Sub
