' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: derived ms-vbal 5.4.3.6 a variable-length String member is a pointer, not data, so the byte image of such a type is refused instead of copying a reference
' ERROR: unsupported
Private Type A
    s As String
End Type
Private Type B
    n As Long
End Type
Public Sub Main()
    Dim x As A, y As B
    LSet y = x
    Debug.Print "reached"
End Sub
