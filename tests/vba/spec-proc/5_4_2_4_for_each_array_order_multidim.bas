' INV: vbal.5.4.2.4.1
' SOURCE: spec ms-vbal 5.4.2.4.1 array enumeration increments the leftmost dimension fastest
' EXPECT: 11 21 12 22 13 23
Public Sub Main()
    Dim a(1 To 2, 1 To 3) As Long
    Dim i As Long, j As Long, v As Variant, s As String
    For i = 1 To 2
        For j = 1 To 3
            a(i, j) = i * 10 + j
        Next j
    Next i
    For Each v In a
        s = s & CStr(v) & " "
    Next v
    Debug.Print Trim$(s)
End Sub
