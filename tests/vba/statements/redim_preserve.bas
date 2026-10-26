' INV: stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 ReDim statement Preserve
' EXPECT:  11  22  0 
' EXPECT:  0 
Public Sub Main()
    Dim a() As Long
    ReDim a(1 To 2)
    a(1) = 11: a(2) = 22
    ReDim Preserve a(1 To 3)
    Debug.Print a(1); a(2); a(3)
    ReDim a(1 To 2)
    Debug.Print a(1)
End Sub
