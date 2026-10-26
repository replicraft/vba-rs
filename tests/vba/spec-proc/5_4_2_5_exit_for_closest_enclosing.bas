' INV: vbal.5.4.2.5, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.5 Exit For leaves the closest enclosing For or For Each, even from within a Do
' EXPECT: i=1 j=2
' EXPECT: i=2 j=2
' EXPECT: done
Public Sub Main()
    Dim i As Long, j As Long, v As Variant
    Dim a(0 To 3) As Long
    For i = 1 To 2
        For Each v In a
            j = j + 1
            If j Mod 2 = 0 Then Exit For
        Next v
        Debug.Print "i=" & CStr(i) & " j=" & CStr(j)
        j = 0
    Next i
    j = 0
    For i = 1 To 5
        Do
            j = j + 1
            Exit For
        Loop
    Next i
    Debug.Print "done"
End Sub
