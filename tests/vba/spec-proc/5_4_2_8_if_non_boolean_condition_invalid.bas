' INV: vbal.5.4.2.8, stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.8 the condition is a boolean-expression (UDT is not Let-coercible to Boolean)
' ERROR: load
Type Pt
    X As Long
End Type
Public Sub Main()
    Dim p As Pt
    If p Then
        Debug.Print "x"
    End If
End Sub
