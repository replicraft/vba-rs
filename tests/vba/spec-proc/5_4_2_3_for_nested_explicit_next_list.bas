' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 explicit-for-statement: Next j, i closes nested loops
' EXPECT: 11 12 21 22
Public Sub Main()
    Dim i As Long, j As Long, s As String
    For i = 1 To 2
        For j = 1 To 2
            s = s & CStr(i) & CStr(j) & " "
    Next j, i
    Debug.Print Trim$(s)
End Sub
