' INV: op.concat
' SOURCE: derived repeated doubling reaches the string limit and stops with a limit fault
' ERROR: limit
Public Sub Main()
    Dim s As String, i As Long
    s = "ab"
    For i = 1 To 40
        s = s & s
    Next
End Sub
