' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 For Each over an array visits all elements; variable ends with the last one
' EXPECT: 5,6,7
' EXPECT: last=7
Public Sub Main()
    Dim a(5 To 7) As Long
    Dim k As Long, v As Variant, s As String
    For k = 5 To 7
        a(k) = k
    Next k
    For Each v In a
        If Len(s) > 0 Then s = s & ","
        s = s & CStr(v)
    Next v
    Debug.Print s
    Debug.Print "last=" & CStr(v)
End Sub
