' INV: vbal.6.1.3.2.1.1
' SOURCE: spec ms-vbal 6.1.3.2.1.1 Err.Clear is called automatically by an On Error statement
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: On Error does not clear Err
' EXPECT: raised=5
' EXPECT: afterResumeNext=0
' EXPECT: raised2=5
' EXPECT: afterGoTo0=0
Public Sub Main()
    On Error Resume Next
    Err.Raise 5
    Debug.Print "raised=" & CStr(Err.Number)
    On Error Resume Next
    Debug.Print "afterResumeNext=" & CStr(Err.Number)
    Err.Raise 5
    Debug.Print "raised2=" & CStr(Err.Number)
    On Error GoTo 0
    Debug.Print "afterGoTo0=" & CStr(Err.Number)
End Sub
