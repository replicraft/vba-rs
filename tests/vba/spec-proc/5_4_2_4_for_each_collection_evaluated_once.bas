' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 the collection expression is evaluated once before the loop
' EXPECT: calls=1 items=a,b,c
Dim calls As Long
Function Get3() As Collection
    calls = calls + 1
    Dim c As New Collection
    c.Add "a"
    c.Add "b"
    c.Add "c"
    Set Get3 = c
End Function
Public Sub Main()
    Dim v As Variant, s As String
    For Each v In Get3()
        If Len(s) > 0 Then s = s & ","
        s = s & v
    Next v
    Debug.Print "calls=" & CStr(calls) & " items=" & s
End Sub
