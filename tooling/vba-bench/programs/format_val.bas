Public Sub Main()
    Dim i As Long, t As Double, s As String
    For i = 1 To 6000
        s = Format(i * 1.37, "0.00")
        t = t + Val(s) + Val(Format(i, "000000"))
        s = Format(i / 7, "#,##0.000")
    Next
    Debug.Print t; s
End Sub
