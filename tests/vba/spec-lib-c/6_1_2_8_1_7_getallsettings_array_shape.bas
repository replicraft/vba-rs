' INV: vbal.6.1.2.8.1.7, func.getallsettings
' SOURCE: spec ms-vbal 6.1.2.8.1.7 GetAllSettings array is two-dimensional, lower bounds 1, upper bounds count and 2
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: GetAllSettings returns a zero-based array
' EXPECT: lb1=1 ub1=2
' EXPECT: lb2=1 ub2=2
' EXPECT: pairs=True
Public Sub Main()
    Dim s As Variant, i As Long, ok As Boolean
    SaveSetting "VbarsConfApp2", "Sec", "Alpha", "1"
    SaveSetting "VbarsConfApp2", "Sec", "Beta", "2"
    s = GetAllSettings("VbarsConfApp2", "Sec")
    Debug.Print "lb1=" & LBound(s, 1) & " ub1=" & UBound(s, 1)
    Debug.Print "lb2=" & LBound(s, 2) & " ub2=" & UBound(s, 2)
    ok = True
    For i = 1 To 2
        If s(i, 1) = "Alpha" And s(i, 2) <> "1" Then ok = False
        If s(i, 1) = "Beta" And s(i, 2) <> "2" Then ok = False
        If s(i, 1) <> "Alpha" And s(i, 1) <> "Beta" Then ok = False
    Next i
    Debug.Print "pairs=" & ok
    DeleteSetting "VbarsConfApp2"
End Sub
