' INV: stmt.redim
' SOURCE: derived an oversized ReDim fails with a classified error (default max_array_elements)
' ERROR: 7
Public Sub Main()
    Dim a() As Long
    ReDim a(1 To 100000000)
End Sub
