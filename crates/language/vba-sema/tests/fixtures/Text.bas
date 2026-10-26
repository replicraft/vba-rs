Attribute VB_Name = "Text"
Option Explicit
Const msg As String = "a" & vbCrLf

Public Function Words(ByVal s As String) As Long
    Dim parts() As String
    Dim i As Long, n As Long
    parts = Split(s, " ")
    For i = LBound(parts) To UBound(parts)
        If Len(Trim$(parts(i))) > 0 Then n = n + 1
    Next i
    Words = n
End Function

Public Sub Dump(ByVal v As Variant, Optional ByVal indent As Long = 0)
    Dim k As Variant
    Select Case TypeName(v)
        Case "Dictionary"
            For Each k In v.Keys
                Dump v(k), indent + 2
            Next k
        Case Else
            Debug.Print Space$(indent) & CStr(v)
    End Select
    If IsMissing(v) Then Exit Sub
End Sub

Public Sub Misc()
    Dim d As Object, x As Variant, arr As Variant, s As String, n As Long
    Set d = CreateObject("Scripting.Dictionary")
    d.Add "a", 1
    d("b") = 2
    arr = Array(1, 2, 3)
    n = UBound(arr) - LBound(arr) + 1
    x = IIf(n > 1, "many", "one")
    s = Format(Now, "yyyy-mm-dd") & " " & Format$(n, "000")
    MsgBox "x" & s, vbInformation + vbOKOnly, "t"
    On Error Resume Next
    n = CLng("abc")
    If Err.Number <> 0 Then
        Debug.Print Err.Description
        Err.Clear
    End If
    On Error GoTo 0
    s = Mid$(s, 2, 3)
    Mid$(s, 1, 1) = "x"
    n = InStr(1, s, "x", vbTextCompare)
    x = Array("a", "b")(1)
    n = Len(msg) + Len(s)
End Sub
