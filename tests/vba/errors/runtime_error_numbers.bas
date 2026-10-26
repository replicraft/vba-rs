' INV: stmt.on-error
' SOURCE: spec ms-vbal 5.6.9.3.5 division by zero 11, 5.6.2.1 subscript 9, 5.5.1.2.4 type mismatch 13, 5.6.3 Nothing 91, 5.5.1.2.1 overflow 6
' EXPECT:  11 
' EXPECT:  9 
' EXPECT:  13 
' EXPECT:  91 
' EXPECT:  6 
Function Try(ByVal k As Long) As Long
    Dim a(1 To 2) As Long, o As Object, n As Long, v As Variant
    On Error Resume Next
    Select Case k
        Case 1: n = 1 / 0
        Case 2: n = a(3)
        Case 3: n = CInt("x")
        Case 4: o.Foo
        Case 5: n = CByte(300)
        Case 6: v = Null: n = v + Len(v)
    End Select
    Try = Err.Number
End Function
Public Sub Main()
    Dim i As Long
    For i = 1 To 5
        Debug.Print Try(i)
    Next
End Sub
