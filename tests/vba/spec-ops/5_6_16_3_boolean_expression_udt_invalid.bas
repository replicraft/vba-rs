' INV: vbal.5.6.16.3, stmt.if-then-else
' SOURCE: spec ms-vbal 5.6.16.3 boolean expression invalid when Let-coercion of its declared type to Boolean is invalid (UDT)
' ERROR: load
Private Type P
    x As Long
End Type
Public Sub Main()
    Dim u As P
    If u Then Debug.Print "x"
End Sub
