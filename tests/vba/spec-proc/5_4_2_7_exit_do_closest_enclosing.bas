' INV: vbal.5.4.2.7, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.7 Exit Do leaves the closest enclosing Do (also when a For lies between)
' EXPECT: outer=3 inner=1
Public Sub Main()
    Dim outer As Long, inner As Long, i As Long
    Do
        outer = outer + 1
        inner = 0
        Do
            inner = inner + 1
            For i = 1 To 3
                Exit Do
            Next i
        Loop
        If outer = 3 Then Exit Do
    Loop
    Debug.Print "outer=" & CStr(outer) & " inner=" & CStr(inner)
End Sub
