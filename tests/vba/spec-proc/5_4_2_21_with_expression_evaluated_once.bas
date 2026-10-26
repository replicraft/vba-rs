' INV: vbal.5.4.2.21, stmt.with
' SOURCE: spec ms-vbal 5.4.2.21 a value/function expression is evaluated once to an anonymous block variable
' EXPECT: calls=1 v=7
'#MODULE Box class
Public V As Long
'#MODULE Module1 standard
Dim calls As Long
Function MakeBox() As Box
    calls = calls + 1
    Set MakeBox = New Box
End Function
Public Sub Main()
    Dim r As Long
    With MakeBox()
        .V = 5
        .V = .V + 2
        r = .V
    End With
    Debug.Print "calls=" & CStr(calls) & " v=" & CStr(r)
End Sub
