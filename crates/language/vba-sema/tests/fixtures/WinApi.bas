Attribute VB_Name = "WinApi"
Option Explicit

#If VBA7 Then
    Private Declare PtrSafe Function GetTickCount Lib "kernel32" () As Long
    Private Declare PtrSafe Sub Sleep Lib "kernel32" (ByVal dwMilliseconds As Long)
    Private Declare PtrSafe Function FindWindow Lib "user32" Alias "FindWindowA" _
        (ByVal lpClassName As String, ByVal lpWindowName As String) As LongPtr
    Private Declare PtrSafe Function SendMessage Lib "user32" Alias "SendMessageA" _
        (ByVal hWnd As LongPtr, ByVal wMsg As Long, ByVal wParam As LongPtr, lParam As Any) As LongPtr
    Public Declare PtrSafe Sub CopyMem Lib "kernel32" Alias "RtlMoveMemory" _
        (Destination As Any, Source As Any, ByVal Length As LongPtr)
#End If

Public Type RECT
    Left As Long
    Top As Long
    Right As Long
    Bottom As Long
End Type

Public Function Elapsed(ByVal since As Long) As Long
    Elapsed = GetTickCount() - since
End Function

Public Function WindowExists(ByVal title As String) As Boolean
    Dim h As LongPtr
    h = FindWindow(vbNullString, title)
    WindowExists = (h <> 0)
End Function

Public Sub Pause(ByVal ms As Long)
    Sleep ms
End Sub
